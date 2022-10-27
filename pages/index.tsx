import React, {useEffect, useMemo, useState} from "react";
import Head from "next/head";
import {
    ActionIcon,
    Button,
    Divider,
    Group,
    Paper,
    SimpleGrid,
    Space,
    Stack,
    Text,
    Title,
    useMantineTheme
} from "@mantine/core";
import {Calendar} from '@mantine/dates';
import {useListState, useScrollIntoView} from "@mantine/hooks";
import 'dayjs/locale/ro'
import {
    GameTable, GuestInvite,
    Location,
    LocationName,
    Profile,
    Reservation,
    ReservationRestriction,
    ReservationStatus
} from "../types/wrapper";
import {supabase} from "../utils/supabase_utils";
import {useAuth} from "../components/AuthProvider";
import {useRouter} from "next/router";
import {MdOutlineNoAccounts, MdRefresh, MdVpnKey} from "react-icons/md";
import {addDaysToDate, dateToISOString, isWeekend} from "../utils/date";
import ConfirmSelection from "../components/ConfirmSelection";
import {Room, SelectedTable} from "../types/room";
import guestInvite from "../components/GuestInvite";

interface IParams {
    gara: Room
    boromir: Room
    daysAhead: number
}

export default function MakeReservationPage(params: IParams): JSX.Element {
    const theme = useMantineTheme()
    const auth = useAuth()
    const router = useRouter()
    const [locationName, /*setLocationName*/] = useState(LocationName.Gara)
    const [selectedDate, setSelectedDate] = useState<Date | null>(null)
    const [selectedTable, setSelectedTable] = useState<SelectedTable | null>(null)

    const selectedDateISO = useMemo(() => {
        if (selectedDate == null)
            return null
        return dateToISOString(selectedDate)
    }, [selectedDate])

    function onSelectedDateChange(selectedDate: Date) {
        setSelectedDate(selectedDate)
        setSelectedTable(null)
    }

    useEffect(() => {
        if (!auth.isLoading && auth.user != null && auth.profile == null) {
            router.push('/signup').then(() => {
                console.log("Failed to redirect to signup")
            })
        }
    }, [auth, router])

    const room = locationName == LocationName.Gara ? params.gara : params.boromir;

    return <>
        <Head>
            <title>Rezervări</title>
        </Head>

        <Title>Rezervări</Title>

        <Space h="lg"/>

        <Paper>
            {(!auth.isLoading && auth.user == null) &&
                <>
                    <Paper shadow={"0"} p={"md"} sx={(theme) => ({
                        backgroundColor: theme.colors.orange,
                        marginTop: theme.spacing.sm
                    })}>
                        <Text>Trebuie să ai un cont și să fi un membru pentru a putea face rezervări!</Text>
                    </Paper>
                    <Space h="md"/>
                </>
            }

            <SimpleGrid
                cols={1}
                breakpoints={[
                    {minWidth: 1120, cols: 2},
                ]}>

                {!auth.isLoading && auth.user != null &&
                    <Stack key={"calendar"}>
                        {/*<Radio.Group
                            value={locationName}
                            onChange={(value) => {
                                switch (value) {
                                    case LocationName.Gara.toString():
                                        setLocationName(LocationName.Gara);
                                        break;
                                    case LocationName.Boromir.toString():
                                        setLocationName(LocationName.Boromir);
                                        break;
                                }
                            }}
                            label={"Alege locația unde faci rezervarea:"}
                            size="md">
                            <Radio value={LocationName.Gara} label={"Gară"}/>
                            <Radio value={LocationName.Boromir} label={"Boromir"}/>
                        </Radio.Group>

                        <Space h={"sm"}/>*/}

                        <Text>Alege ziua rezervării:</Text>

                        <Calendar
                            minDate={new Date}
                            maxDate={addDaysToDate(new Date, params.daysAhead)}
                            hideOutsideDates={true}
                            allowLevelChange={false}
                            size={"lg"}
                            locale={"ro"}
                            value={selectedDate}
                            onChange={(date) => {
                                if (auth.user != null && date != null)
                                    onSelectedDateChange(date)
                            }}
                            dayStyle={(date) =>
                                (date.getDate() === (new Date).getDate()
                                    && date.getMonth() === (new Date).getMonth()
                                    && date.getDate() !== selectedDate?.getDate())
                                    ? {backgroundColor: theme.colors.blue[4], color: theme.white} : {}
                            }
                            fullWidth={true}
                        />
                    </Stack>
                }

                <Stack>
                    {SelectGameTable(room, selectedDateISO, selectedTable, setSelectedTable)}

                    {ConfirmSelection(room, selectedDateISO, selectedTable)}
                </Stack>
            </SimpleGrid>
        </Paper>
    </>;
}

function fetchReservations(
    setReservations: (data: Reservation[]) => void,
    setRestrictions: (data: ReservationRestriction[]) => void
) {
    supabase.from<Reservation>('rezervari')
        .select('*')
        .gte('start_date', dateToISOString(new Date))
        .eq('status', ReservationStatus.Approved)
        .order('created_at', {ascending: true})
        .then(value => {
            if (value.data != null) {
                setReservations(value.data)
                console.log("Reservations updated")
            }
        })

    supabase.from<ReservationRestriction>('reservations_restrictions')
        .select('*')
        .gte('date', dateToISOString(new Date))
        .then(value => {
            if (value.data != null) {
                setRestrictions(value.data)
                console.log("Restrictions updated")
            }
        })
}

function SelectGameTable(
    room: Room,
    selectedDateISO: string | null,
    selectedTable: SelectedTable | null,
    onSelectTable: (s: SelectedTable) => void,
): JSX.Element {

    const [reservations, reservationsHandle] = useListState<Reservation>([])
    const [allProfiles, setAllProfiles] = useState<Profile[]>([])
    const [restrictions, setRestrictions] = useState<ReservationRestriction[]>([])
    const [invites, setInvites] = useState<GuestInvite[]>([])
    const {scrollIntoView, targetRef} = useScrollIntoView<HTMLDivElement>({});

    useEffect(() => {
        supabase.from<Profile>('profiles').select('*').then(value => {
            if (value.data != null) {
                setAllProfiles(value.data)
            }
        })

        fetchReservations(reservationsHandle.setState, setRestrictions);

        supabase.from<GuestInvite>('guest_invites')
            .select('*')
            .then(value => {
                if (value.data !== null)
                    setInvites(value.data)
            })

        const subscription = supabase.from<Reservation>('rezervari')
            .on('INSERT', payload => {
                console.log(payload.new)
                if (payload.new.status == ReservationStatus.Approved) {
                    reservationsHandle.setState(prev => [...prev, payload.new]
                        .sort((a, b) => { // @ts-ignore
                            return new Date(a.created_at) - new Date(b.created_at)
                        }))
                }
            })
            .on('UPDATE', payload => {
                fetchReservations(reservationsHandle.setState, setRestrictions) // TODO Could make this more efficient
            })
            .on('DELETE', payload => {
                reservationsHandle.filter(value => value.id != payload.old.id)
            })
            .subscribe()

        return () => {
            subscription?.unsubscribe()
        }
        // eslint-disable-next-line react-hooks/exhaustive-deps
    }, [])

    const currentDateReservations = useMemo(() => {
        return reservations.filter(reservation => reservation.start_date == selectedDateISO)
    }, [reservations, selectedDateISO]);

    const currentDateInvites = useMemo(() => {
        return invites.filter(invite => invite.date == selectedDateISO)
    }, [invites, selectedDateISO]);

    useEffect(() => {
        if (selectedDateISO) {
            scrollIntoView({alignment: 'center'})
        }
        // eslint-disable-next-line react-hooks/exhaustive-deps
    }, [selectedDateISO])

    const startEnd = useMemo(() => {
        if (selectedDateISO == null) {
            return {start: 0, end: 0, duration: 0}
        }
        if (isWeekend(new Date(selectedDateISO))) {
            return {
                start: room.weekendStartHour,
                end: room.weekendEndHour,
                duration: room.weekendDuration,
            }
        } else {
            return {
                start: room.startHour,
                end: room.endHour,
                duration: room.duration,
            }
        }
    }, [room, selectedDateISO])

    if (selectedDateISO == null) return <></>

    const selectedTableId = selectedTable?.table?.id;
    const selectedStartHour = (selectedTable) ? selectedTable.startHour : -1;

    const tableButtons = function (startHour: number) {
        return room.tables.map((gameTable) => {
            return (
                <Button
                    variant={(gameTable.id == selectedTableId && startHour == selectedStartHour) ? "filled" : "outline"}
                    key={gameTable.id}
                    fullWidth={false}
                    onClick={() => onSelectTable(new SelectedTable(startHour, gameTable))}>
                    {gameTable.name}
                </Button>
            )
        })
    }

    const allButtons = ({start, end, duration}) => {
        let content: JSX.Element[] = [];
        for (let hour = start; hour < end; hour += duration) {
            const restriction =
                restrictions.find(restriction => restriction.date == selectedDateISO && restriction.start_hour == hour)

            content.push(<Stack key={hour}>
                <Group noWrap={true} style={{marginLeft: "1em", marginRight: "1em"}} spacing={'lg'}>
                    <Text>{`Ora ${hour} - ${hour + duration}`}:</Text>

                    {!restriction &&
                        tableButtons(hour)
                    }

                    {restriction &&
                        <Text>{restriction.message}</Text>
                    }
                </Group>

                {!restriction &&
                    <Group style={{marginLeft: "1em", marginRight: "1em"}} spacing={"xs"}>
                        <Text>Listă înscriși: </Text>
                        {currentDateReservations.filter(value => value.start_hour == hour).map((reservation, index) => {
                            const profile = allProfiles.find(value => value.id == reservation.user_id)

                            if (profile)
                                return <Button key={profile.id} color={profile.has_key ? 'blue' : 'gray'} radius={'xl'}
                                               size={'xs'} rightIcon={profile.has_key ?
                                    <MdVpnKey/> : <></>}>{index + 1}. {profile.name}</Button>
                            else
                                return <></>
                        })}

                        {currentDateInvites.filter(value => value.start_hour == hour).map((invite) => {
                            return <Button key={invite.date + invite.start_hour} color={'cyan'} radius={'xl'}
                                           size={'xs'} rightIcon={<MdOutlineNoAccounts/>}>{invite.guest_name}</Button>
                        })}
                    </Group>
                }

                <Divider variant={"dashed"}/>
            </Stack>);
        }
        return content;
    };

    return <>
        <Group position={'apart'} align={"center"}>
            <Group align={"center"} spacing={'xs'}>
                <Text weight={600} ref={targetRef}>Data selectată:</Text>
                <Text color={"blue"}>{(new Date(selectedDateISO)).toLocaleDateString('ro-RO')}</Text>
            </Group>

            <ActionIcon variant={'light'} radius={'xl'} size={36}
                        onClick={() => fetchReservations(reservationsHandle.setState, setRestrictions)}>
                <MdRefresh size={28}/>
            </ActionIcon>
        </Group>

        {allButtons(startEnd)}
    </>
}


export async function getStaticProps({}) {
    function locationToRoom(location: Location, tables: GameTable[]): Room {
        return {
            locationName: location.name as LocationName,
            tables,
            maxReservations: location!.max_reservations,
            startHour: location!.start_hour,
            endHour: location!.end_hour,
            duration: location!.reservation_duration,
            weekendStartHour: location!.weekend_start_hour,
            weekendEndHour: location!.weekend_end_hour,
            weekendDuration: location!.weekend_reservation_duration,
        }
    }

    const {data: gameTables} = await supabase.from<GameTable>('mese').select('*')
    const garaTables = gameTables!.filter((value) => value.location == LocationName.Gara)
    const boromirTables = gameTables!.filter((value) => value.location == LocationName.Boromir)

    const {data: locations} = await supabase.from<Location>('locations').select('*')
    const garaLocation = locations!.find(value => value.name == LocationName.Gara)
    const boromirLocation = locations!.find(value => value.name == LocationName.Boromir)

    const props: IParams = {
        daysAhead: 14,
        gara: locationToRoom(garaLocation!, garaTables),
        boromir: locationToRoom(boromirLocation!, boromirTables)
    }

    return {props}
}
