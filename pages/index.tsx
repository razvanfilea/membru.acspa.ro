import React, {useEffect, useMemo, useState} from "react";
import Head from "next/head";
import {
    ActionIcon,
    Button,
    Card,
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
import {useScrollIntoView} from "@mantine/hooks";
import {NextLink} from "@mantine/next";
import ReservationComponent from "../components/Reservation";
import 'dayjs/locale/ro'
import {GameTable, Location, LocationName, Profile, Reservation, ReservationStatus} from "../types/wrapper";
import {supabase} from "../utils/supabase_utils";
import {useAuth} from "../components/AuthProvider";
import {useRouter} from "next/router";
import {MdRefresh, MdVpnKey} from "react-icons/md";
import {addDaysToDate, dateToISOString} from "../utils/date";

interface IParams {
    gara: Room
    boromir: Room
    daysAhead: number
}

interface Room {
    locationName: LocationName
    tables: GameTable[]
    maxReservations: number
    startHour: number
    endHour: number
    duration: number
    weekendStartHour: number
    weekendEndHour: number
    weekendDuration: number
}

class SelectedTable {
    readonly startHour: number
    readonly table: GameTable

    constructor(startHour: number, table: GameTable) {
        this.startHour = startHour
        this.table = table
    }
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
    })

    const room = locationName == LocationName.Gara ? params.gara : params.boromir;

    return <>
        <Head>
            <title>Rezervări</title>
            <meta name="viewport" content="initial-scale=1.0, width=device-width"/>
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
                    {minWidth: 860, cols: 2},
                ]}>
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

                    {!auth.user != null &&
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
                    }
                </Stack>

                <Stack>
                    {SelectGameTable(room, selectedDateISO, selectedTable, setSelectedTable)}

                    {ConfirmSelection(room, selectedDateISO, selectedTable)}
                </Stack>
            </SimpleGrid>
        </Paper>
    </>;
}

function fetchReservations(setReservations: (data: Reservation[]) => void) {
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
}

function SelectGameTable(room: Room,
                         selectedDateISO: string | null,
                         selectedTable: SelectedTable | null,
                         onSelectTable: (s: SelectedTable) => void): JSX.Element {

    const [reservations, setReservations] = useState<Reservation[]>([])
    const [allProfiles, setAllProfiles] = useState<Profile[]>([])
    const {scrollIntoView, targetRef} = useScrollIntoView<HTMLDivElement>({});

    useEffect(() => {
        supabase.from<Profile>('profiles').select('*').then(value => {
            if (value.data != null) {
                setAllProfiles(value.data)
            }
        })

        fetchReservations(setReservations);

        const subscription = supabase.from<Reservation>('rezervari')
            .on('INSERT', payload => {
                console.log(payload.new)
                if (payload.new.status == ReservationStatus.Approved) {
                    setReservations(prev => [...prev, payload.new]
                        .sort((a, b) => { // @ts-ignore
                            return new Date(a.created_at) - new Date(b.created_at)
                        }))
                }
            })
            .on('UPDATE', payload => {
                fetchReservations(setReservations) // TODO Could make this more efficient
            })
            .on('DELETE', payload => {
                setReservations(prev => prev.filter(value => value.id != payload.old.id))
            })
            .subscribe()

        return () => {
            subscription?.unsubscribe()
        }
    }, [])

    const currentDateReservations = useMemo(() => {
        return reservations.filter(reservation => reservation.start_date == selectedDateISO)
    }, [reservations, selectedDateISO]);

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
        const date = new Date(selectedDateISO)
        if (date.getDay() === 6 || date.getDay() === 0) {
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
            content.push(<Stack key={hour}>
                <Group noWrap={true} style={{marginLeft: "1em", marginRight: "1em"}} spacing={'lg'}>
                    <Text>{`Ora ${hour} - ${hour + duration}`}:</Text>

                    {tableButtons(hour)}
                </Group>

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
                </Group>

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

            <ActionIcon variant={'light'} radius={'xl'} size={36} onClick={() => fetchReservations(setReservations)}>
                <MdRefresh size={28}/>
            </ActionIcon>
        </Group>
        {allButtons(startEnd)}
    </>
}

function ConfirmSelection(
    room: Room,
    selectedDateISO: string | null,
    selectedTable: SelectedTable | null
): JSX.Element {
    const enum ConfirmationStatus {
        None,
        Loading,
        Success,
        Fail
    }

    const [status, setStatus] = useState(ConfirmationStatus.None)
    const [errorMessage, setErrorMessage] = useState("")
    const {scrollIntoView, targetRef} = useScrollIntoView<HTMLDivElement>({});

    useEffect(() => {
        if (selectedDateISO && selectedTable) {
            scrollIntoView({alignment: 'center'})
            setStatus(ConfirmationStatus.None)
            setErrorMessage("")
        }
        // eslint-disable-next-line react-hooks/exhaustive-deps
    }, [selectedDateISO, selectedTable])

    if (selectedDateISO == null || selectedTable == null) return <></>

    // Fake Reservation to display to user
    const fakeReservation: Reservation = {
        id: '',
        created_at: '',
        user_id: '',
        table_id: selectedTable.table.id,
        start_date: selectedDateISO,
        start_hour: selectedTable.startHour,
        duration: room.duration,
        status: ReservationStatus.Approved
    }

    function DisplayConfirmationStatus(): JSX.Element {
        if (status == ConfirmationStatus.None || status == ConfirmationStatus.Loading) {
            return <Button
                fullWidth style={{marginTop: 14}}
                loading={status == ConfirmationStatus.Loading}
                color={'blue'}
                onClick={async () => {
                    setStatus(ConfirmationStatus.Loading)
                    const reservationParams = {
                        table_id_input: selectedTable!.table.id,
                        start_date_input: selectedDateISO,
                        start_hour_input: selectedTable!.startHour,
                    }
                    const errorMessage = await publishReservation(reservationParams)
                    setStatus(errorMessage == null ? ConfirmationStatus.Success : ConfirmationStatus.Fail)
                    setErrorMessage(errorMessage ?? "")
                }}>Confirmă rezervarea!</Button>
        } else if (status == ConfirmationStatus.Success) {
            return <Stack>
                <Paper shadow={"0"} p={"md"} sx={(theme) => ({
                    backgroundColor: theme.colors.green,
                    marginTop: theme.spacing.sm,
                    marginBottom: theme.spacing.xs
                })}>
                    <Text align={"center"} color="#FFF"><b>Rezervarea ta a fost înregistrată</b></Text>
                </Paper>

                <Group align={"center"}>
                    <Text weight={600}>Poți anula oricând această rezervare de pe pagina ta de profil:</Text>
                    <NextLink href={"/profile"}>
                        <Button variant={'light'}>Vezi profilul</Button>
                    </NextLink>
                </Group>
            </Stack>
        } else {
            return <Paper shadow={"0"} p={"md"} sx={(theme) => ({
                backgroundColor: theme.colors.orange,
                marginTop: theme.spacing.sm,
                marginBottom: theme.spacing.xs
            })}>
                <Text align={"center"} color="#FFF">Rezervarea nu a putut fi realizată: {errorMessage}</Text>
            </Paper>
        }
    }

    return (<Card p={"xl"} shadow={"sm"}>
        <div style={{marginTop: 'sm'}}>
            <Title order={2} ref={targetRef}>Confirmă rezervarea:</Title>
        </div>

        <Space h={"lg"}/>

        {ReservationComponent(fakeReservation, selectedTable.table, false, async () => {
        })}

        <Space h={"md"}/>

        {DisplayConfirmationStatus()}

    </Card>)
}

async function publishReservation(reservationParams): Promise<string | null> {
    console.log(reservationParams)

    const {data, error} = await supabase.rpc('create_reservation', reservationParams)

    if (error != null) return "a fost întâmpinată o eroare (" + error.message + ")"

    // @ts-ignore TypeScript is actually wrong here
    return data == '' ? null : data // An empty string means no errors
}

export async function getStaticProps({}) {
    const {data: gameTables} = await supabase.from<GameTable>('mese').select('*')
    const garaTables = gameTables!.filter((value) => value.location == LocationName.Gara)
    const boromirTables = gameTables!.filter((value) => value.location == LocationName.Boromir)

    const {data: locations} = await supabase.from<Location>('locations').select('*')
    const garaLocation = locations!.find(value => value.name == LocationName.Gara)
    const boromirLocation = locations!.find(value => value.name == LocationName.Boromir)

    const props: IParams = {
        daysAhead: 14,
        gara: {
            locationName: LocationName.Gara,
            tables: garaTables,
            maxReservations: garaLocation!.max_reservations,
            startHour: garaLocation!.start_hour,
            endHour: garaLocation!.end_hour,
            duration: garaLocation!.reservation_duration,
            weekendStartHour: garaLocation!.weekend_start_hour,
            weekendEndHour: garaLocation!.weekend_end_hour,
            weekendDuration: garaLocation!.weekend_reservation_duration,
        },
        boromir: {
            locationName: LocationName.Boromir,
            tables: boromirTables,
            startHour: boromirLocation!.start_hour,
            maxReservations: boromirLocation!.max_reservations,
            endHour: boromirLocation!.end_hour,
            duration: boromirLocation!.reservation_duration,
            weekendStartHour: boromirLocation!.weekend_start_hour,
            weekendEndHour: boromirLocation!.weekend_end_hour,
            weekendDuration: boromirLocation!.end_hour,
        }
    }

    return {props}
}
