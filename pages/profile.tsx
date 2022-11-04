import {
    ActionIcon,
    Avatar,
    Button,
    Card,
    Center,
    Checkbox,
    Group,
    Loader,
    Paper,
    Stack,
    Text,
    Title
} from "@mantine/core";
import React, {useEffect, useMemo, useState} from "react";
import {NextRouter, useRouter} from "next/router";
import ReservationComponent from "../components/Reservation";
import {useProfile} from "../components/ProfileProvider";
import {GameTable, Profile, Reservation, ReservationStatus} from "../types/wrapper";
import {MdRefresh} from "react-icons/md";
import {isReservationCancelable} from "../utils/date";
import {Database} from "../types/database.types";
import {SupabaseClient, useSupabaseClient} from "@supabase/auth-helpers-react";
import {useListState} from "@mantine/hooks";
import {createBrowserSupabaseClient} from "@supabase/auth-helpers-nextjs";

interface IParams {
    gameTables: GameTable[]
}

async function signOut(supabase: SupabaseClient<Database>, router: NextRouter) {
    const {error} = await supabase.auth.signOut()

    if (error == null) {
        await router.push("/login")
    } else {
        console.log('Auth', error);
        console.log(error); // Failure
    }
}

function fetchReservations(
    supabase: SupabaseClient<Database>,
    profile: Profile,
    setReservations: (data: Reservation[]) => void,
) {
    supabase.from('rezervari')
        .select('*')
        .eq("user_id", profile.id)
        .order('start_date', {ascending: false})
        .order('start_hour', {ascending: true})
        .then(res => {
            setReservations(res.data || [])
        })
}

export default function ProfilePage(params: IParams) {
    const supabase = useSupabaseClient<Database>()
    const router = useRouter()
    const profileData = useProfile()

    const [reservations, reservationsHandle] = useListState<Reservation>([])
    const [showCancelled, setShowCancelled] = useState(false)

    useEffect(() => {
        if (!profileData.isLoading && profileData.profile == null)
            router.push('/login').then(() => {
            })
    }, [profileData, router])

    useEffect(() => {
        if (profileData.profile == null)
            return;

        fetchReservations(supabase, profileData.profile, reservationsHandle.setState)
        // We only want to run it once
        // eslint-disable-next-line react-hooks/exhaustive-deps
    }, [])

    const filteredReservations = useMemo(() => {
        return reservations.filter((res) => res.status == ReservationStatus.Approved || showCancelled)
    }, [reservations, showCancelled])

    if (profileData.isLoading)
        return <Center> <Loader/> </Center>;

    if (profileData.profile == null)
        return (<></>)

    return (<>
        <Paper shadow={"md"} p={'lg'} sx={(theme) => ({
            backgroundColor: theme.colorScheme === 'dark' ? theme.colors.dark[4] : theme.colors.gray[1],
            margin: theme.spacing.lg,
            '@media (max-width: 900px)': {
                margin: theme.spacing.xs,
            },
        })}>

            <Group position={"apart"}>
                <Group noWrap={true}>
                    <Avatar
                        src={`https://ui-avatars.com/api/?name=${profileData.profile.name}&background=random&rounded=true`}
                        radius={"md"}/>

                    <Stack spacing={1}>
                        <Text size="md" weight={500}>{profileData.profile.name}</Text>
                        <Text size="sm">{profileData.profile.role}</Text>
                    </Stack>
                </Group>

                <Button variant={"filled"} color={'red'} onClick={() => signOut(supabase, router)}>Sign out</Button>
            </Group>

        </Paper>

        <Stack sx={(theme) => ({
            padding: theme.spacing.lg,
            '@media (max-width: 900px)': {
                paddingLeft: theme.spacing.md,
                paddingRight: theme.spacing.md,
            },
            '@media (max-width: 600px)': {
                paddingLeft: 0,
                paddingRight: 0,
            }
        })}>
            <Group position={'apart'}>
                <Title order={2}>Rezervările tale:</Title>

                <ActionIcon variant={'light'} radius={'xl'} size={36}
                            onClick={() => fetchReservations(supabase, profileData.profile!, reservationsHandle.setState)}>
                    <MdRefresh size={28}/>
                </ActionIcon>
            </Group>

            <Checkbox label="Afișează anulate"
                      checked={showCancelled}
                      onChange={(event) => setShowCancelled(event.currentTarget.checked)}/>

            {reservations.length == 0 &&
                <Text size={"lg"}>Nu ați făcut nicio rezervare</Text>
            }

            {filteredReservations.map((reservation) => (
                <Card key={reservation.id}>
                    {ReservationComponent(
                        reservation,
                        params.gameTables.find((element) => element.id == reservation.table_id)!,
                        true,
                        (isReservationCancelable(reservation)) ? (async () => {
                            const newData = {
                                ...reservation,
                                status: ReservationStatus.Cancelled
                            }

                            const {data} = await supabase.from('rezervari').update(newData).select()
                            reservationsHandle.filter(value => value.id != data![0].id)
                            reservationsHandle.append(data![0])
                        }) : null
                    )}
                </Card>
            ))}
        </Stack>
    </>)
}

export async function getStaticProps({}) {
    const supabase = createBrowserSupabaseClient<Database>()
    const {data: gameTables} = await supabase.from('mese').select('*')

    const props: IParams = {
        gameTables: gameTables!
    }

    return {
        props
    }
}
