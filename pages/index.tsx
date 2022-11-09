import React, {Suspense, useEffect, useMemo, useState} from "react";
import Head from "next/head";
import {ActionIcon, Group, Paper, SimpleGrid, Space, Stack, Text, Title, useMantineTheme} from "@mantine/core";
import {useListState, useLocalStorage, useScrollIntoView} from "@mantine/hooks";
import 'dayjs/locale/ro'
import {
    GameTable,
    GuestInvite,
    Location,
    LocationName,
    Profile,
    Reservation,
    ReservationRestriction,
    ReservationStatus
} from "../types/wrapper";
import {useProfile} from "../components/ProfileProvider";
import {useRouter} from "next/router";
import {MdClose, MdRefresh} from "react-icons/md";
import {addDaysToDate, dateToISOString, isDateWeekend} from "../utils/date";
import ConfirmSelection from "../components/ConfirmSelection";
import {Room, SelectedTable} from "../types/room";
import dynamic from "next/dynamic";
import {SupabaseClient, useSession, useSupabaseClient} from "@supabase/auth-helpers-react";
import {Database} from "../types/database.types";
import {createBrowserSupabaseClient} from "@supabase/auth-helpers-nextjs";
import RegistrationHours from "../components/RegistrastationHours";

const DynamicCalendar = dynamic(() => import('../components/MantineCalendar'), {
    suspense: true,
    ssr: true,
})

interface IParams {
    gara: Room
    boromir: Room
    daysAhead: number
}

interface IShowInformationPopup {
    readonly value: boolean
    readonly expiry: number
}

export default function MakeReservationPage(params: IParams): JSX.Element {
    const router = useRouter()
    const theme = useMantineTheme()
    const session = useSession()
    const profileData = useProfile()

    const [locationName, /*setLocationName*/] = useState(LocationName.Gara)
    const [selectedDate, setSelectedDate] = useState<Date>(new Date)
    const [selectedTable, setSelectedTable] = useState<SelectedTable | null>(null)

    const selectedDateISO = useMemo(() => dateToISOString(selectedDate), [selectedDate])

    function onSelectedDateChange(selectedDate: Date) {
        setSelectedDate(selectedDate)
        setSelectedTable(null)
    }

    useEffect(() => {
        if (!profileData.isLoading && profileData.profile == null) {
            const redirectPath = session == null ? '/login' : '/create_profile'
            const timer = setTimeout(() => {
                router.push(redirectPath).then(null)
            }, 400)

            return () => clearTimeout(timer)
        }
    }, [session, profileData, router])

    const [showInformationPopup, setInformationPopup] = useLocalStorage<IShowInformationPopup>({
        key: 'show-info-popup',
        defaultValue: {
            value: true,
            expiry: new Date().getTime() - 1000
        },
        getInitialValueInEffect: true,
    })


    const room = locationName == LocationName.Gara ? params.gara : params.boromir;

    return <>
        <Head>
            <title>Rezervări</title>
        </Head>

        <Title>Rezervări</Title>

        <Space h="lg"/>

        <Paper>
            {(showInformationPopup.value || showInformationPopup.expiry < new Date().getTime()) &&
                <>
                    <Paper shadow={"0"} p={"sm"} sx={(theme) => ({
                        backgroundColor: theme.colors.cyan[9],
                    })}>
                        <Group noWrap={true}>
                            <Text style={{width: '100%'}}>
                                Rezervările se fac până la ora 16 pentru ziua respectivă. Max 8 jucători pentru un
                                interval orar. Când știți că nu ajungeți, retrageți-vă pentru a lăsa loc liber altor
                                jucători. Rezervările se fac până la ora 16 pentru ziua în curs. Spor la joc!</Text>
                            <ActionIcon onClick={() => {
                                const daysInMilliseconds = 3 * 24 * 60 * 60 * 10000 // 3 days in milliseconds
                                const item: IShowInformationPopup = {
                                    value: false,
                                    expiry: new Date().getTime() + daysInMilliseconds
                                }

                                setInformationPopup(item)
                            }} size={48}>
                                <MdClose size={24}/>
                            </ActionIcon>
                        </Group>
                    </Paper>
                    <Space h="md"/>
                </>
            }

            <SimpleGrid
                cols={1}
                breakpoints={[
                    {minWidth: 1120, cols: 2},
                ]}>

                {!profileData.isLoading && profileData.profile != null &&
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

                        <Suspense fallback={`Loading Calendar...`}>
                            <DynamicCalendar
                                minDate={new Date}
                                maxDate={addDaysToDate(new Date, params.daysAhead)}
                                hideOutsideDates={true}
                                allowLevelChange={false}
                                size={"lg"}
                                locale={"ro"}
                                value={selectedDate}
                                onChange={(date) => {
                                    if (profileData.profile != null && date != null)
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
                        </Suspense>
                    </Stack>
                }

                <Stack>
                    {SelectGameTable(room, selectedDateISO, selectedTable, setSelectedTable)}

                    {ConfirmSelection(room, selectedDateISO, selectedTable)}
                </Stack>
            </SimpleGrid>

            <Space h="xl"/>
        </Paper>
    </>;
}

function fetchReservations(
    supabase: SupabaseClient<Database>,
    setReservations: (data: Reservation[]) => void,
    setRestrictions: (data: ReservationRestriction[]) => void
) {
    supabase.from('rezervari')
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

    supabase.from('reservations_restrictions')
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
    selectedDateISO: string,
    selectedTable: SelectedTable | null,
    onSelectTable: (s: SelectedTable) => void,
): JSX.Element {
    const supabase = useSupabaseClient<Database>()

    const [reservations, reservationsHandle] = useListState<Reservation>([])
    const [allProfiles, setAllProfiles] = useState<Profile[]>([])
    const [restrictions, setRestrictions] = useState<ReservationRestriction[]>([])
    const [invites, setInvites] = useState<GuestInvite[]>([])
    const {scrollIntoView, targetRef} = useScrollIntoView<HTMLDivElement>({});

    useEffect(() => {
        supabase.from('profiles').select('*').then(value => {
            if (value.data != null) {
                setAllProfiles(value.data)
            }
        })

        fetchReservations(supabase, reservationsHandle.setState, setRestrictions);

        supabase.from('guest_invites')
            .select('*')
            .then(value => {
                if (value.data !== null)
                    setInvites(value.data)
            })

        const reservationListener = supabase.channel('rezervari')
            .on(
                'postgres_changes',
                {event: '*', schema: 'public', table: 'rezervari'},
                (payload) => {
                    if (payload.eventType == "INSERT") {
                        if (payload.new.status == ReservationStatus.Approved) {
                            // @ts-ignore // TODO
                            reservationsHandle.setState(prev => [...prev, payload.new]
                                .sort((a, b) => {
                                    // @ts-ignore
                                    return new Date(a.created_at) - new Date(b.created_at)
                                }))
                        }
                    } else if (payload.eventType == "UPDATE") {
                        fetchReservations(supabase, reservationsHandle.setState, setRestrictions) // TODO Could make this more efficient
                    } else {
                        reservationsHandle.filter(value => value.id != payload.old.id)
                    }
                }
            )
            .subscribe()

        return () => {
            reservationListener?.unsubscribe()
        }
        // eslint-disable-next-line react-hooks/exhaustive-deps
    }, [])

    // eslint-disable-next-line react-hooks/exhaustive-deps
    useEffect(() => scrollIntoView({alignment: 'center'}), [selectedDateISO])

    const registrationHours = useMemo(() => {
        if (selectedDateISO == null) {
            return {start: 0, end: 0, duration: 0}
        }
        if (isDateWeekend(new Date(selectedDateISO))) {
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

    const selectedDateReservations
        = useMemo(() => reservations.filter(value => value.start_date == selectedDateISO), [reservations, selectedDateISO])
    const selectedDateInvites
        = useMemo(() => invites.filter(value => value.date == selectedDateISO), [invites, selectedDateISO])
    const selectedDateRestrictions
        = useMemo(() => restrictions.filter(value => value.date == selectedDateISO), [restrictions, selectedDateISO])

    return <>
        <Group position={'apart'} align={"center"}>
            <Group align={"center"} spacing={'xs'}>
                <Text weight={600} ref={targetRef}>Data selectată:</Text>
                <Text color={"blue"}>{new Date(selectedDateISO).toLocaleDateString('ro-RO')}</Text>
            </Group>

            <ActionIcon variant={'light'} radius={'xl'} size={36}
                        onClick={() => fetchReservations(supabase, reservationsHandle.setState, setRestrictions)}>
                <MdRefresh size={28}/>
            </ActionIcon>
        </Group>

        {RegistrationHours(room.tables, selectedDateReservations, selectedDateRestrictions, selectedDateInvites, allProfiles, selectedTable, onSelectTable, registrationHours)}
    </>
}


export async function getStaticProps({}) {
    const supabase = createBrowserSupabaseClient<Database>()

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

    const {data: gameTables} = await supabase.from('mese').select('*')
    const garaTables = gameTables!.filter((value) => value.location == LocationName.Gara)
    const boromirTables = gameTables!.filter((value) => value.location == LocationName.Boromir)

    const {data: locations} = await supabase.from('locations').select('*')
    const garaLocation = locations!.find(value => value.name == LocationName.Gara)
    const boromirLocation = locations!.find(value => value.name == LocationName.Boromir)

    const props: IParams = {
        daysAhead: 14,
        gara: locationToRoom(garaLocation!, garaTables),
        boromir: locationToRoom(boromirLocation!, boromirTables)
    }

    return {props}
}
